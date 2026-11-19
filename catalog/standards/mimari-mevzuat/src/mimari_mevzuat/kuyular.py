"""Şaftlar ve asansörler: Binaların Yangından Korunması Hakkında Yönetmelik 25/3, 41/9, 62/2, 62/4, 62/6, 62/7 ve Ek-3/B
12 (docs/adr/8087, 8092, 8135).

- **25/3:** yüksek binada (4/ccc: bina yüksekliği 21,50 m'den ya da yapı yüksekliği 30,50 m'den fazla) çöp, haberleşme,
  evrak ve teknik donanım gibi düşey tesisat şaftı ve bacanın duvarları en az 120, kapakları en az 90 dakika dayanıklı ve
  duman sızdırmaz. Duvar E ve I ile en az 120; kapak (şaftın duvarındaki kapı, pencere, boşluk) E ile en az 90 ve S.
  Açık yer şaftı bozar.
- **41/9:** kaçış merdiveni yuvasına ve yangın güvenlik holüne elektrik ve mekanik tesisat şaftı kapağı açılamaz: şaftın
  kapısının öbür yanı kaçış merdiveni mahali ya da yangın güvenlik holü olmamalı. Çöp bacası tesisat şaftı sayılmaz.
- **62/2:** asansör kuyusu yangına en az 60 dakika dayanıklı: duvarları E ve I ile en az 60, duvarındaki pencere ya da
  boşluk da öyle; açık yer kuyuyu bozar. Kuyunun kapısı asansörün kat kapısıdır (62/7). "Yanıcı olmayan malzeme" ve
  makine dairesi çizimde okunmaz.
- **62/4:** asansör kuyusunda en az 0,1 m² olmak üzere kuyu alanının 0,025 katı kadar havalandırma ve dumandan
  arındırma bacası bulunur ya da kuyu basınçlandırılır (docs/adr/8135): bir kuyu katlarca tek kuyudur (``stack``), alanı
  katlarındaki en büyük alanı, beyanı katlarından birinin; katlarında farklı beyan uzman incelemesidir. Asansör
  kapıları koridor, hol ve benzeri alanlar dışında doğrudan kullanım alanlarına açılamaz: kapının bir yanı geçit
  (koridor, hol, antre, giriş, lobi, sahanlık, merdiven mahali) olmalı. Bodruma da inen asansöre bodrum katta korunmuş
  koridordan (4/dd, 36/1-a: bitişik mahallerden Ek-3/C'nin bodrum süresince EI ile ayrılmış) ya da yangın güvenlik
  holünden ulaşılır (docs/adr/8092).
- **62/6:** asansör kapısı yangın merdiveni yuvasına açılamaz: kapının bir yanı adında Yangın merdiveni ya da Korunumlu
  merdiven geçen mahal olmamalı.
- **62/7 (9.7.2015):** asansör kat kapısı Ek-3/B 12'nin dayanımında: yapı yüksekliği 51,50 m'den yüksek binada E 60,
  alçak binada E 30; tam 51,50 m'de iki satır da tutmaz (uzman incelemesi). Önceki metni kural setinde yok.

Beyan yoksa ya da okunmuyorsa eksik veri, yetmiyorsa kaldı.

**Okunan olgular** (``facts.building``): şaftlar (``shafts``: tür, ad, kat, çevresindeki duvar ve açıklıklar, beyan
edilen sınıfları ve boyları, açık boy; alanı, katlarca hangi kuyu olduğu, asansör kuyusunun beyan edilen bacası ya da
basınçlandırılması), kapılar (``doors``: asansör kapısı mı, bir yanı geçit mi, kaçış merdiveni ya da
yangın güvenlik holü mü, iki yanındaki mahaller, beyan edilen sınıfı), mahallerin adları, bina ve yapı yüksekliği.
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import dayanim
from . import katman
from . import yururluk as y
from .yangin_sinifi import ayirir, okuma

MADDELER = [("BYKHY", "25/3"), ("BYKHY", "41/9"), ("BYKHY", "62/2"), ("BYKHY", "62/4"), ("BYKHY", "62/6"),
            ("BYKHY", "62/7")]

#: Bir açık yer bundan kısaysa sayılmaz (çizimin köşe gürültüsü), m.
_EN_AZ = 0.05

_TUR = {"wall": "duvar", "door": "kapı", "window": "pencere", "gap": "boşluk"}


def _katla(metin: str) -> str:
    tablo = str.maketrans({"ç": "C", "Ç": "C", "ğ": "G", "Ğ": "G", "ı": "I", "İ": "I", "i": "I", "ö": "O", "Ö": "O",
                           "ş": "S", "Ş": "S", "ü": "U", "Ü": "U", "â": "A", "Â": "A", "î": "I", "Î": "I"})
    return metin.translate(tablo).upper()


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


def _sonuc(hal: y.Hal, bulgular: list[dict[str, Any]], kaldi: str, eksik: str, gecti: str
           ) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """Maddenin satırı bulgularından: biri kaldıysa kaldı, yalnız eksik veri ya da uzman incelemesiyse o, yoksa geçti."""
    if any("state" not in b for b in bulgular):
        return bulgular, _satir(hal, y.KALDI, kaldi)
    if any(b.get("state") == y.EKSIK_VERI for b in bulgular):
        return bulgular, _satir(hal, y.EKSIK_VERI, eksik)
    if bulgular:
        return bulgular, _satir(hal, y.UZMAN, eksik)
    return bulgular, _satir(hal, y.GECTI, gecti)


def _tutar(beyan: str | None, harfler: str, dakika: int, duman: bool = False) -> bool | None:
    """Beyan istenen harflerle en az ``dakika`` (``duman``: S'li) mı; beyan yoksa ya da okunmuyorsa yok."""
    s = okuma(beyan)
    if s is None:
        return None
    return all(h in s["harf"] for h in harfler) and s["dakika"] >= dakika and (not duman or s["duman"])


def saftlar(bina: dict[str, Any], tur: str) -> list[dict[str, Any]]:
    """Binanın ``tur`` (``service``, ``elevator``) şaftları."""
    return [s for s in bina.get("shafts") or [] if s.get("kind") == tur]


def asansor_kapilari(bina: dict[str, Any]) -> list[dict[str, Any]]:
    """Bir yanı asansör kuyusu olan kapılar."""
    return [k for k in bina.get("doors") or [] if k.get("elevator")]


def yuksek(bina: dict[str, Any], hal: y.Hal) -> tuple[bool, str]:
    """Yüksek bina mı (4/ccc) ve sözü."""
    h, s = float(bina.get("height") or 0), float(bina.get("structureHeight") or 0)
    soz = f"bina yüksekliği {h:.2f} m, yapı yüksekliği {s:.2f} m"
    return h > float(hal.anahtar["building"]) + 1e-9 or s > float(hal.anahtar["structure"]) + 1e-9, soz


def _sorunlar(saft: dict[str, Any], duvar: tuple[str, int], kapak: tuple[str, int, bool] | None
              ) -> tuple[list[str], list[str], bool]:
    """Bir şaftın tutmayanları türüne, sorununa ve beyanına göre toplanmış: sözleri, nesneleri ve yalnız beyansızlıktan
    mı."""
    # Çevresinde hiçbir duvar ya da açıklık yoksa çizim eksiktir (yalnız mahal çizilmiş): açık sayılmaz.
    if not saft.get("walls") and not saft.get("openings"):
        return ["çevresinde duvar çizilmemiş"], [], True
    gruplar: dict[tuple[str, str, str, str], list[str]] = {}
    ogeler = [(w, duvar[0], duvar[1], False) for w in saft.get("walls") or []]
    if kapak is not None:
        ogeler += [(o, kapak[0], kapak[1], kapak[2]) for o in saft.get("openings") or []]
    for o, harf, dk, duman in ogeler:
        t = _tutar(o.get("fireRating"), harf, dk, duman)
        if t:
            continue
        istek = f"en az {harf} {dk}" + (", duman sızdırmaz" if duman else "")
        sorun = "az" if t is False else ("okunmadı" if o.get("fireRating") else "yok")
        anahtar = (_TUR.get(o.get("kind"), "eleman"), sorun, o.get("fireRating") or "", istek)
        gruplar.setdefault(anahtar, []).append(o["uid"])
    neler, uids = [], []
    for (tur, sorun, beyan, istek), u in gruplar.items():
        uids += u
        if sorun == "az":
            neler.append(f"{len(u)} {tur} {beyan} ({istek})")
        elif sorun == "okunmadı":
            neler.append(f"{len(u)} {tur} “{beyan}” okunamadı ({istek})")
        else:
            neler.append(f"{len(u)} {tur} sınıfı beyan edilmemiş ({istek})")
    eksik_mi = all(k[1] != "az" for k in gruplar)
    if float(saft.get("open") or 0) > _EN_AZ:
        eksik_mi = False
        neler.append(f"{float(saft['open']):.2f} m açık (duvar yok)")
    return neler, uids, eksik_mi


def tesisat(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """25/3: yüksek binada düşey tesisat şaftı ve baca."""
    hepsi = saftlar(bina, "service")
    if not hepsi:
        return [], _satir(hal, y.KAPSAM_DISI, "düşey tesisat şaftı ya da baca yok")
    evet, soz = yuksek(bina, hal)
    if not evet:
        return [], _satir(hal, y.KAPSAM_DISI, f"yüksek bina değil ({soz})")
    duvar = ("EI", int(hal.anahtar["walls"]))
    kapak = ("E", int(hal.anahtar["covers"]), True)
    bulgular = []
    for s in hepsi:
        neler, uids, eksik_mi = _sorunlar(s, duvar, kapak)
        if neler:
            bulgular.append(_bulgu(
                "bykhy_shaft_resistance",
                f"“{s.get('name') or 'Şaft'}”: " + "; ".join(neler) + f"; 25/3 yüksek binada ({soz}) şaft ve baca "
                f"duvarının en az 120, kapağının en az 90 dakika dayanıklı ve duman sızdırmaz olmasını ister",
                "Şaftın duvarlarını en az EI 120, kapaklarını en az E 90 ve duman sızdırmaz (S) yapın ve sınıflarını "
                "beyan edin (Öznitelikler › Yangın dayanımı).", hal, [s["uid"]] + uids, f"{s['uid']}:tesisat",
                y.EKSIK_VERI if eksik_mi else None))
    kalan = sum(1 for b in bulgular if "state" not in b)
    return _sonuc(hal, bulgular, f"{kalan} şaft 25/3'ü tutmuyor ({soz})",
                  "şaft duvarlarının ya da kapaklarının sınıfı beyan edilmemiş",
                  f"{len(hepsi)} şaft en az EI 120, kapakları en az E 90 S ({soz})")


def kapaklar(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """41/9: tesisat şaftının kapağı kaçış merdiveni yuvasına ya da yangın güvenlik holüne açılmaz."""
    hepsi = [s for s in saftlar(bina, "service") if "COP" not in _katla(s.get("name") or "")]
    if not hepsi:
        return [], _satir(hal, y.KAPSAM_DISI, "elektrik ya da mekanik tesisat şaftı yok")
    kapilar = {k["uid"]: k for k in bina.get("doors") or []}
    adlar = {m["uid"]: m.get("name") or "Mahal" for m in bina.get("spaces") or []}
    bulgular = []
    for s in hepsi:
        for o in s.get("openings") or []:
            k = kapilar.get(o["uid"])
            if k is None or not (k.get("stair") or k.get("lobby")):
                continue
            yer = "kaçış merdiveni yuvasına" if k.get("stair") else "yangın güvenlik holüne"
            odalar = [u for u in k.get("between") or [] if u]
            ad = ", ".join(f"“{adlar.get(u, 'Mahal')}”" for u in odalar)
            bulgular.append(_bulgu(
                "bykhy_shaft_cover_stair",
                f"Şaft kapısı (“{s.get('name') or 'Şaft'}”) {yer} açılıyor" + (f" ({ad})" if ad else "") +
                ": 41/9 kaçış merdiveni yuvasına ve yangın güvenlik holüne elektrik ve mekanik tesisat şaftı kapağının "
                "açılmasına izin vermez",
                "Şaftın kapağını bir koridora, hole ya da teknik hacme alın.", hal, [k["uid"], s["uid"]] + odalar,
                f"{k['uid']}:kapak"))
    return _sonuc(hal, bulgular, f"{len(bulgular)} şaft kapağı kaçış merdivenine ya da yangın güvenlik holüne açılıyor",
                  "", f"{len(hepsi)} şaftın kapağı kaçış merdivenine ya da yangın güvenlik holüne açılmıyor")


def kuyu(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """62/2: asansör kuyusu en az 60 dakika."""
    hepsi = saftlar(bina, "elevator")
    if not hepsi:
        return [], _satir(hal, y.KAPSAM_DISI, "asansör kuyusu yok")
    dk = int(hal.anahtar["minutes"])
    bulgular = []
    for s in hepsi:
        # Kuyunun kapısı asansörün kat kapısıdır (62/7); öbür açıklıklar kuyunun duvarı gibi okunur.
        delikli = dict(s, openings=[o for o in s.get("openings") or [] if o.get("kind") != "door"])
        neler, uids, eksik_mi = _sorunlar(delikli, ("EI", dk), ("EI", dk, False))
        if neler:
            bulgular.append(_bulgu(
                "bykhy_elevator_shaft",
                f"“{s.get('name') or 'Asansör'}”: " + "; ".join(neler) + f"; 62/2 asansör kuyusunun yangına en az {dk} "
                f"dakika dayanıklı olmasını ister",
                f"Kuyunun duvarlarını en az EI {dk} yapın ve sınıfını beyan edin; kuyu duvarındaki pencere ya da boşluğu "
                f"kapatın ya da en az EI {dk} yapın.", hal, [s["uid"]] + uids, f"{s['uid']}:kuyu",
                y.EKSIK_VERI if eksik_mi else None))
    kalan = sum(1 for b in bulgular if "state" not in b)
    return _sonuc(hal, bulgular, f"{kalan} asansör kuyusu {dk} dakikayı tutmuyor",
                  "kuyu duvarlarının sınıfı beyan edilmemiş", f"{len(hepsi)} asansör kuyusu en az EI {dk}")


def _bodrum_katlari(bina: dict[str, Any]) -> set[str]:
    """Zemin katın altındaki katlar (adı Zemin olan, yoksa ±0.00'a en yakın kat zemindir)."""
    katlar = list(bina.get("storeys") or [])
    if not katlar:
        return set()
    zemin = next((k for k in katlar if "ZEMIN" in _katla(k.get("name") or "")), None) or min(
        katlar, key=lambda k: abs(katman.kot(k)))
    return {k["level"] for k in katlar if float(k.get("elevation") or 0) < float(zemin.get("elevation") or 0) - 1e-9}


def _m2(x: float) -> str:
    """Bir alanın sözü, m²: en çok üç ondalık, sondaki sıfırlar atılmış (0.11, 0.105, 4.4)."""
    return f"{x:.3f}".rstrip("0").rstrip(".")


def havalandirma(bina: dict[str, Any], hal: y.Hal) -> list[dict[str, Any]]:
    """62/4'ün ilk cümlesi (docs/adr/8135): asansör kuyusunda en az 0,1 m² olmak üzere kuyu alanının 0,025 katı kadar
    havalandırma ve dumandan arındırma bacası bulunur ya da kuyu basınçlandırılır. Katlarca tek kuyu (``stack``) bir
    kez okunur: alanı katlarındaki en büyük alanı, beyanı katlarından birinin. Beyan ya da alan yoksa eksik veri,
    katlarında farklı beyan varsa uzman incelemesi, baca istenenden küçükse kaldı."""
    en_az, pay = float(hal.anahtar["ventLeast"]), float(hal.anahtar["ventShare"])
    kuyular_: dict[str, list[dict[str, Any]]] = {}
    for s in saftlar(bina, "elevator"):
        kuyular_.setdefault(s.get("stack") or s["uid"], []).append(s)
    soz = (f"62/4 asansör kuyusunda en az {_m2(en_az)} m² olmak üzere kuyu alanının {pay} katı kadar havalandırma ve "
           f"dumandan arındırma bacası bulunmasını ya da kuyunun basınçlandırılmasını ister")
    bulgular = []
    for yigin, uyeler in kuyular_.items():
        ad = uyeler[0].get("name") or "Asansör"
        uids = [s["uid"] for s in uyeler]
        beyanlar: list[dict[str, Any]] = []
        for s in uyeler:
            v = s.get("vent")
            if v and v not in beyanlar:
                beyanlar.append(v)
        alan = max(float(s.get("area") or 0) for s in uyeler)
        istek = max(en_az, pay * alan)
        olcu = f"kuyu alanı {_m2(alan)} m², en az {_m2(istek)} m² baca" if alan > 0 else "kuyunun alanı ölçülemedi"
        onarim = (f"Kuyunun havalandırma ve dumandan arındırma bacasını en az {_m2(istek)} m² yapın ya da kuyuyu "
                  f"basınçlandırın ve beyan edin (asansör kuyusu mahalinin Öznitelikler'inde Baca alanı, "
                  f"Basınçlandırılır).")
        iz = f"{yigin}:baca"
        if len(beyanlar) > 1:
            bulgular.append(_bulgu(
                "bykhy_elevator_shaft_vent", f"“{ad}”: kuyunun katlarında farklı havalandırma beyanları var ({olcu}); {soz}",
                "Kuyunun havalandırma beyanını bir katında bırakın ya da bütün katlarında aynı yapın.", hal, uids, iz,
                y.UZMAN))
            continue
        v = beyanlar[0] if beyanlar else None
        if v is not None and v.get("pressurized"):
            continue
        baca = v.get("area") if v is not None else None
        if baca is None:
            bulgular.append(_bulgu("bykhy_elevator_shaft_vent",
                                   f"“{ad}”: kuyunun havalandırma bacası ya da basınçlandırılması beyan edilmemiş "
                                   f"({olcu}); {soz}", onarim, hal, uids, iz, y.EKSIK_VERI))
        elif alan <= 0:
            bulgular.append(_bulgu("bykhy_elevator_shaft_vent",
                                   f"“{ad}”: havalandırma bacası {_m2(float(baca))} m², ama kuyunun alanı ölçülemedi; "
                                   f"{soz}", "Kuyuyu mahal olarak kapalı bir dış çizgiyle çizin.", hal, uids, iz,
                                   y.EKSIK_VERI))
        elif float(baca) + 1e-9 < istek:
            bulgular.append(_bulgu("bykhy_elevator_shaft_vent",
                                   f"“{ad}”: havalandırma bacası {_m2(float(baca))} m² ({olcu}); {soz}", onarim, hal,
                                   uids, iz))
    return bulgular


def bodrum_holu(bina: dict[str, Any], hal: y.Hal, gun: date) -> list[dict[str, Any]]:
    """62/4'ün ikinci cümlesi (docs/adr/8092): bodruma da inen asansöre bodrum katta korunmuş koridordan ya da yangın
    güvenlik holünden ulaşılır. Korunmuş koridor (4/dd, 36/1-a): bitişik mahallerden Ek-3/C'nin bodrum süresince EI ile
    ayrılmış, açık yeri olmayan koridor ya da hol. Önü geçit olmayan kapının bulgusu 62/4'ün ilk cümlesinindir."""
    bodrum = _bodrum_katlari(bina)
    kapilar_ = [k for k in asansor_kapilari(bina) if k.get("level") in bodrum and k.get("passage") and not k.get("lobby")]
    if not kapilar_:
        return []
    adlar = {m["uid"]: m.get("name") or "Mahal" for m in bina.get("spaces") or []}
    katlar = {k["level"]: k.get("name") or k["level"] for k in bina.get("storeys") or []}
    dk, soz = dayanim.bodrum_suresi(bina, gun)
    istek = ("62/4 bodruma da inen asansöre bodrum katta korunmuş koridordan ya da yangın güvenlik holünden "
             "ulaşılmasını ister")
    onarim = ("Asansörün bodrum kattaki önünü yangın güvenlik holü yapın ya da koridorun bitişik mahallerle ayrımlarını "
              "(duvar, kapı) Ek-3/C'nin bodrum süresince EI beyan edin; açık yer bırakmayın.")
    bulgular = []
    for k in kapilar_:
        odalar = [u for u in k.get("between") or [] if u]
        yer = katlar.get(k.get("level") or "", "Bodrum")
        ad = ", ".join(f"“{adlar.get(u, 'Mahal')}”" for u in odalar)
        if dk is None:
            bulgular.append(_bulgu("bykhy_elevator_basement_lobby",
                                   f"{yer}: asansör kapısının önü ({ad}) korunmuş koridor mu okunmuyor: {soz}; {istek}",
                                   onarim, hal, [k["uid"]] + odalar, f"{k['uid']}:bodrum", y.EKSIK_VERI))
            continue
        ayrimlar = [a for a in bina.get("separations") or [] if any(u in a["rooms"] for u in odalar)]
        acik = [a for a in ayrimlar if float(a.get("open") or 0) > _EN_AZ]
        sonuclar = [ayirir(o.get("fireRating"), dk) for a in ayrimlar
                    for o in list(a.get("walls") or []) + list(a.get("openings") or [])]
        if acik or any(s is False for s in sonuclar):
            neden = "açık yeri var" if acik else f"ayrımı en az EI {dk} değil"
            bulgular.append(_bulgu("bykhy_elevator_basement_lobby",
                                   f"{yer}: asansör kapısı korunmamış koridora açılıyor ({ad}, {neden}); {istek} "
                                   f"(36/1-a: {soz}, en az EI {dk})", onarim, hal, [k["uid"]] + odalar,
                                   f"{k['uid']}:bodrum"))
        elif any(s is None for s in sonuclar):
            bulgular.append(_bulgu("bykhy_elevator_basement_lobby",
                                   f"{yer}: asansör kapısının önündeki koridorun ({ad}) ayrımlarının sınıfı beyan "
                                   f"edilmemiş; {istek} (36/1-a: {soz}, en az EI {dk})", onarim, hal,
                                   [k["uid"]] + odalar, f"{k['uid']}:bodrum", y.EKSIK_VERI))
    return bulgular


def onu(bina: dict[str, Any], hal: y.Hal, gun: date) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """62/4: asansör kuyusunun havalandırma bacası ya da basınçlandırılması; asansör kapısı kullanım alanına değil,
    koridor, hol ve benzerine açılır; bodruma inen asansöre bodrumda korunmuş koridordan ya da yangın güvenlik holünden
    ulaşılır."""
    hepsi = asansor_kapilari(bina)
    kuyular_ = {s.get("stack") or s["uid"] for s in saftlar(bina, "elevator")}
    if not hepsi and not kuyular_:
        return [], _satir(hal, y.KAPSAM_DISI, "asansör kuyusu ve kapısı yok")
    adlar = {m["uid"]: m.get("name") or "Mahal" for m in bina.get("spaces") or []}
    bulgular = havalandirma(bina, hal)
    for k in hepsi:
        if k.get("passage"):
            continue
        odalar = [u for u in k.get("between") or [] if u]
        if not odalar:
            bulgular.append(_bulgu(
                "bykhy_elevator_door_room",
                "Asansör kapısının önünde mahal çizilmemiş: 62/4 asansör kapısının koridor, hol ve benzeri bir alana "
                "açılmasını ister", "Asansörün önündeki holü ya da koridoru mahal olarak çizin ve adlandırın.", hal,
                [k["uid"]], f"{k['uid']}:önü", y.EKSIK_VERI))
            continue
        ad = ", ".join(f"“{adlar.get(u, 'Mahal')}”" for u in odalar)
        bulgular.append(_bulgu(
            "bykhy_elevator_door_room",
            f"Asansör kapısı kullanım alanına açılıyor ({ad}): 62/4 asansör kapılarının koridor, hol ve benzeri alanlar "
            f"dışında doğrudan kullanım alanlarına açılmamasını ister",
            "Asansörün önüne bir hol ya da koridor çizin (adı Hol, Koridor, Lobi, Sahanlık) ya da kapıyı oraya alın.",
            hal, [k["uid"]] + odalar, f"{k['uid']}:önü"))
    bulgular += bodrum_holu(bina, hal, gun)
    baca = [b for b in bulgular if b["code"] == "bykhy_elevator_shaft_vent"]
    kapi = [b for b in bulgular if b["code"] != "bykhy_elevator_shaft_vent"]
    kaldi = []
    if n := sum("state" not in b for b in baca):
        kaldi.append(f"{n} asansör kuyusunun havalandırma bacası yetmiyor")
    if n := sum("state" not in b for b in kapi):
        kaldi.append(f"{n} asansör kapısı kullanım alanına ya da bodrumda korunmamış koridora açılıyor")
    eksik = []
    if any(b.get("state") == y.EKSIK_VERI for b in baca):
        eksik.append("asansör kuyusunun havalandırması beyan edilmemiş")
    if any(b.get("state") == y.UZMAN for b in baca):
        eksik.append("asansör kuyusunun katlarında farklı havalandırma beyanları")
    if any("state" in b for b in kapi):
        eksik.append("asansör kapısının önü okunmuyor")
    gecti = []
    if kuyular_:
        gecti.append(f"{len(kuyular_)} asansör kuyusu havalandırılıyor ya da basınçlandırılıyor")
    if hepsi:
        gecti.append(f"{len(hepsi)} asansör kapısı koridor ya da hole açılıyor")
    return _sonuc(hal, bulgular, "; ".join(kaldi), "; ".join(eksik), "; ".join(gecti))


def _yangin_merdiveni(ad: str) -> bool:
    a = _katla(ad)
    return "MERDIVEN" in a and ("YANGIN" in a or "KORUNUMLU" in a)


def merdivene(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """62/6: asansör kapısı yangın merdiveni yuvasına açılmaz."""
    hepsi = asansor_kapilari(bina)
    if not hepsi:
        return [], _satir(hal, y.KAPSAM_DISI, "asansör kapısı yok")
    adlar = {m["uid"]: m.get("name") or "" for m in bina.get("spaces") or []}
    bulgular = []
    for k in hepsi:
        yuva = [u for u in k.get("between") or [] if u and _yangin_merdiveni(adlar.get(u, ""))]
        if yuva:
            bulgular.append(_bulgu(
                "bykhy_elevator_door_stair",
                f"Asansör kapısı yangın merdiveni yuvasına açılıyor (“{adlar[yuva[0]]}”): 62/6 izin vermez",
                "Asansörü yangın merdiveninin dışındaki bir hole ya da koridora açın.", hal, [k["uid"]] + yuva,
                f"{k['uid']}:yuva"))
    return _sonuc(hal, bulgular, f"{len(bulgular)} asansör kapısı yangın merdiveni yuvasına açılıyor", "",
                  f"{len(hepsi)} asansör kapısı yangın merdiveni yuvasına açılmıyor")


def kat_kapilari(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """62/7 ve Ek-3/B 12: asansör kat kapısının dayanımı yapı yüksekliğine göre."""
    hepsi = asansor_kapilari(bina)
    if not hepsi:
        return [], _satir(hal, y.KAPSAM_DISI, "asansör kapısı yok")
    yapi, sinir = float(bina.get("structureHeight") or 0), float(hal.anahtar["height"])
    if abs(yapi - sinir) < 1e-9:
        b = _bulgu("bykhy_elevator_door_resistance",
                   f"Yapı yüksekliği tam {yapi:.2f} m: Ek-3/B 12'nin 51,50 m'den yüksek (E 60) ve alçak (E 30) binaların "
                   f"satırlarından ikisi de tutmaz", "Asansör kat kapılarının süresine uzman karar versin.", hal,
                   sorted(k["uid"] for k in hepsi), "sınır", y.UZMAN)
        return [b], _satir(hal, y.UZMAN, f"yapı yüksekliği tam {yapi:.2f} m")
    yuksek_ = yapi > sinir
    dk = int(hal.anahtar["high"] if yuksek_ else hal.anahtar["low"])
    soz = f"yapı yüksekliği {yapi:.2f} m (51,50 m'den {'yüksek' if yuksek_ else 'alçak'})"
    gruplar: dict[tuple[str, str], list[str]] = {}
    for k in hepsi:
        t = _tutar(k.get("fireRating"), "E", dk)
        if t:
            continue
        sorun = "az" if t is False else ("okunmadı" if k.get("fireRating") else "yok")
        gruplar.setdefault((sorun, k.get("fireRating") or ""), []).append(k["uid"])
    bulgular = []
    for (sorun, beyan), uids in gruplar.items():
        if sorun == "az":
            ne = f"{len(uids)} asansör kat kapısı {beyan}"
        elif sorun == "okunmadı":
            ne = f"{len(uids)} asansör kat kapısının sınıfı “{beyan}” okunamadı"
        else:
            ne = f"{len(uids)} asansör kat kapısının sınıfı beyan edilmemiş"
        bulgular.append(_bulgu(
            "bykhy_elevator_door_resistance",
            f"{ne}: 62/7 kat kapısının Ek-3/B 12'deki dayanımını ister, {soz} en az E {dk}",
            f"Asansörün kat kapılarını en az E {dk} seçin ve sınıfını beyan edin (Öznitelikler › Yangın dayanımı).", hal,
            sorted(uids), f"kapı:{sorun}:{beyan}:E{dk}", None if sorun == "az" else y.EKSIK_VERI))
    kalan = sum(len(b["uids"]) for b in bulgular if "state" not in b)
    return _sonuc(hal, bulgular, f"{kalan} asansör kat kapısı en az E {dk} değil; {soz}",
                  "asansör kat kapılarının sınıfı beyan edilmemiş", f"{len(hepsi)} asansör kat kapısı en az E {dk}; {soz}")


#: Her maddenin denetimi ve başvuru günü ilk hâlinden önceyse uzman incelemesine gidecek nesneleri.
_DENETIMLER = (
    ("25/3", tesisat, lambda b: [s["uid"] for s in saftlar(b, "service")]),
    ("41/9", kapaklar, lambda b: [s["uid"] for s in saftlar(b, "service")]),
    ("62/2", kuyu, lambda b: [s["uid"] for s in saftlar(b, "elevator")]),
    ("62/4", onu, lambda b: [s["uid"] for s in saftlar(b, "elevator")] + [k["uid"] for k in asansor_kapilari(b)]),
    ("62/6", merdivene, lambda b: [k["uid"] for k in asansor_kapilari(b)]),
    ("62/7", kat_kapilari, lambda b: [k["uid"] for k in asansor_kapilari(b)]),
)


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin şaft ve asansör bulguları."""
    bulgular: list[dict[str, Any]] = []
    satirlar: list[dict[str, Any]] = []
    for madde, denetim, nesneler in _DENETIMLER:
        hal = y.hal("BYKHY", madde, gun)
        if hal is None:
            uids = sorted(nesneler(bina))
            if not uids:
                continue
            ilk = y.haller("BYKHY", madde)[0]
            ileti = (f"Başvuru günü ({y.yaz(gun)}) BYKHY {madde} maddesinin kural setindeki ilk hâlinden ({ilk.etiket}) "
                     f"önce: o günün metni KentOS'ta yok")
            bulgular.append({"code": "regulation_text_unknown", "message": ileti,
                             "repair": "Şaftları ve asansörleri maddenin başvuru günündeki metnine göre uzman incelesin.",
                             "uids": uids[:20], "rule": f"BYKHY {madde}", "_iz": f"{madde}:bilinmiyor",
                             "state": y.UZMAN})
            satirlar.append({"rule": f"BYKHY {madde}", "title": ilk.baslik, "version": None, "gazette": None,
                             "state": y.UZMAN, "note": ileti})
            continue
        b, s = onu(bina, hal, gun) if madde == "62/4" else denetim(bina, hal)
        bulgular += b
        satirlar.append(s)
    return y.Sonuc(bulgular, satirlar)


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin şaft ve asansör bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "şaft ve asansör kuralı")
