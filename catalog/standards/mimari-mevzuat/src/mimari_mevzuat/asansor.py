"""Asansör: Planlı Alanlar İmar Yönetmeliği 34/1 ve 34/8, Sivas İmar Yönetmeliği 34/1 ve 34/8 (docs/adr/8078).

Maddenin her hâli bodrumu başka şeyle sayar (``data/surumler.json``):

- **34/1, 28.7.2018:** uygulama imar planına göre kat adedi 3 olan binada asansör yeri, 4 ve daha fazlasında asansör;
  iskân edilen ve ortak alan bulunan bodrumlar dahil kat adedi 4 ve daha fazlaysa asansör.
- **34/1, 11.7.2021:** aynı; bodrumu iskân edilmesi ya da bağımsız bölüm bulunması saydırır.
- **34/1, 1.7.2026:** bağımsız bölüm, ortak alan (teknik hacimler dışında) ya da eklenti bulunan bodrum dahil kat adedi 3
  olan binada asansör yeri, 4 ve daha fazlasında asansör. Tek bağımsız bölümlü konut her hâlde aranmaz.
- **34/8:** asansör zorunlu binada asansör bodrumlar dahil bütün katlara hizmet verir; 11.7.2021 hâlinde bağımsız bölüm
  ya da iskân edilen alan, 1.7.2026 hâlinde bağımsız bölüm, ortak alan ya da eklenti bulunmayan bodruma ulaşması
  gerekmez. Yalnız teknik hacmi olan bodrum (34/8 ortak alanı ayırmaz, 34/1 teknik hacmi saymaz) uzman incelemesidir.
- **Sivas 34/1 (10.11.2022):** en çok üç katlı tek bağımsız bölümlü konut dışında kat adedi 3 ve daha fazla binada
  asansör; iskân edilen ve ortak alan bulunan bodrum sayılır. **Sivas 34/8:** PAİY'nin 11.7.2021 hâli gibi, her
  asansörde.

**Okunan olgular** (``facts.building``, KentOS'un ölçtüğü): her katın kotu, mahallerinin bina programındaki grubu
(bağımsız bölüm, ortak alan, eklenti, grupsuz), teknik ve iskân edilen mahal sayısı, asansörleri (Asansör adlı mahal ya
da Asansör bloğu); programın bağımsız bölümleri ve konutları. ±0,00 ve üstündeki her kat sayılır (projenin katları);
bodrum içindekine göre sayılır; asansörün ulaşması mahali olan katlarda aranır.
Grupsuz mahal hiçbir bağımsız bölümün değildir: ortak alan gibi okunur. "Uygulama imar planına göre kat adedi" künyenin
``kat_adedi``'dir, yazılmamışsa çiziminki. Yerel yönetmelik daha sıkıysa o bağlar ve PAİY'ninki iletide yanında yazılır;
gevşekse ve çizim yalnız ona uyuyorsa çelişkidir, idare karar verir (PAİY 69/3).
"""

from __future__ import annotations

import re
from datetime import date
from typing import Any

from . import yururluk as y

#: Kuralın maddeleri; yerel yönetmelik seçiliyse onunkiler de.
ULUSAL = [("PAİY", "34/1"), ("PAİY", "34/8")]
YEREL = [("Sivas", "34/1"), ("Sivas", "34/8")]

ONARIM = ("Asansör kuyusunu çizin (boşluk mahal, adı “Asansör”) ya da kitaplığın Asansör bloğunu yerleştirin. Tek "
          "bağımsız bölümlü konutsa Bina programında tek bölümü tanımlayın: o zaman aranmaz.")
ULASMIYOR = ("Asansör kuyusunu bu katta da çizin (boşluk mahal, adı “Asansör”, aynı yerde) ya da bodrumun gruplarını "
             "Bina programında düzeltin.")


def _odali(s: dict[str, Any]) -> bool:
    return (s["units"] + s["common"] + s["annexes"] + s["other"] + s["technical"]) > 0


def _sayilir(s: dict[str, Any], kural: str) -> bool:
    """34/1'in hâlinin bodrumu sayıp saymadığı."""
    if kural == "inhabited-or-common":
        return bool(s["units"] or s["inhabited"] or s["common"] or s["other"] or s["technical"])
    if kural == "inhabited-or-unit":
        return bool(s["units"] or s["inhabited"])
    return bool(s["units"] or s["common"] or s["other"] or s["annexes"])


def _ulasilir(s: dict[str, Any], kural: str) -> bool | str:
    """34/8'in hâlinin asansörü bodruma ulaştırıp ulaştırmadığı; ``"uzman"``: yalnız teknik hacim."""
    if kural == "unit-or-inhabited":
        return bool(s["units"] or s["inhabited"])
    if s["units"] or s["common"] or s["other"] or s["annexes"]:
        return True
    return "uzman" if s["technical"] else False


def _icinde(s: dict[str, Any]) -> str:
    """Katta ne var: “bağımsız bölüm ve eklenti”."""
    parca = []
    if s["units"]:
        parca.append("bağımsız bölüm")
    if s["common"] or s["other"]:
        parca.append("ortak alan")
    if s["annexes"]:
        parca.append("eklenti")
    if s["inhabited"] and not s["units"]:
        parca.append("iskân edilen mahal")
    if s["technical"]:
        parca.append("teknik hacim")
    if not parca:
        return "mahal yok"
    return ", ".join(parca[:-1]) + (" ve " if len(parca) > 1 else "") + parca[-1]


def _plan(permit: dict[str, Any]) -> int | None:
    metin = str((permit.get("particulars") or {}).get("kat_adedi") or "").strip()
    return int(metin) if re.fullmatch(r"\d+", metin) else None


class _Bina:
    """Kuralın okuduğu bina: katları, bodrumları, tek konut olup olmadığı, asansörü."""

    def __init__(self, bina: dict[str, Any], plan: int | None) -> None:
        self.katlar = bina.get("storeys") or []
        self.ust = [s for s in self.katlar if s["elevation"] >= -1e-6]
        self.hizmet = [s for s in self.ust if _odali(s)]
        self.bodrum = [s for s in self.katlar if s["elevation"] < -1e-6]
        self.tek_konut = bina.get("units") == 1 and bina.get("dwellings") == 1
        self.plan = plan
        self.asansor = any(s["elevators"] > 0 for s in self.katlar)

    def gerek(self, hal: y.Hal) -> tuple[str | None, str, str | None]:
        """Hâlin istediği (``asansor``, ``yer`` ya da yok), sayılan katların sözü ve aranmıyorsa nedeni."""
        sayilan = [b for b in self.bodrum if _sayilir(b, hal.anahtar["basements"])]
        plandan = bool(hal.anahtar.get("planStoreys")) and self.plan is not None
        ust = self.plan if plandan else len(self.ust)
        n = ust + len(sayilan)
        soz = (f"bodrum dahil {n} kat" if sayilan else f"{n} kat") + (", kat adedi künyeden" if plandan else "")
        if hal.yonetmelik == "PAİY":
            if self.tek_konut:
                return None, soz, "tek bağımsız bölümlü konutta aranmaz"
            if hal.anahtar.get("planStoreys"):
                gerek = "asansor" if ust >= 4 or n >= 4 else ("yer" if ust == 3 else None)
            else:
                gerek = "asansor" if n >= 4 else ("yer" if n == 3 else None)
        else:
            if self.tek_konut and n <= 3:
                return None, soz, "en çok üç katlı tek bağımsız bölümlü konutta aranmaz"
            gerek = "asansor" if n >= 3 else None
        return gerek, soz, None if gerek else f"{soz}: aranmaz"


def _ne(gerek: str | None, neden: str | None) -> str:
    if gerek == "asansor":
        return "4 ve daha fazla katta asansör"
    if gerek == "yer":
        return "3 katta asansör yeri"
    return neden or "aranmaz"


def _bulgu(code: str, ileti: str, onarim: str, kural: str, iz: str, state: str | None = None,
           uids: list[str] | None = None) -> dict[str, Any]:
    """Bir bulgu; ``uids`` adını verdiği nesneler (katın mahalleri: tabloda seçilir, istisnası onlara bağlanır)."""
    out: dict[str, Any] = {"code": code, "message": ileti, "repair": onarim, "uids": list(uids or []), "rule": kural,
                           "_iz": iz}
    if state:
        out["state"] = state
    return out


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def _bilinmiyor(yonetmelik: str, madde: str, gun: date) -> tuple[dict[str, Any], dict[str, Any]]:
    """Başvuru günü maddenin kural setindeki ilk hâlinden önce: uzman incelemesi."""
    ilk = y.haller(yonetmelik, madde)[0]
    ileti = (f"Başvuru günü ({y.yaz(gun)}) {yonetmelik} {madde}'in kural setindeki ilk hâlinden ({ilk.etiket}) önce: "
             f"o günün metni KentOS'ta yok")
    bulgu = _bulgu("regulation_text_unknown", ileti, "Maddenin başvuru günündeki metnine göre uzman incelesin.",
                   f"{yonetmelik} {madde}", f"{madde}:bilinmiyor", y.UZMAN)
    satir = {"rule": f"{yonetmelik} {madde}", "title": ilk.baslik, "version": None, "gazette": None, "state": y.UZMAN,
             "note": ileti}
    return bulgu, satir


def kural(bina: _Bina, sivas: bool, gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin asansör bulguları ve maddelerin satırları."""
    bulgular: list[dict[str, Any]] = []
    satirlar: list[dict[str, Any]] = []
    h1, h8 = y.hal("PAİY", "34/1", gun), y.hal("PAİY", "34/8", gun)
    s1 = y.hal("Sivas", "34/1", gun) if sivas else None
    s8 = y.hal("Sivas", "34/8", gun) if sivas else None
    ulusal = yerel = None
    soz_u = soz_y = ""
    neden_u = None
    if h1 is None:
        b, s = _bilinmiyor("PAİY", "34/1", gun)
        bulgular.append(b)
        satirlar.append(s)
    else:
        ulusal, soz_u, neden_u = bina.gerek(h1)
    if s1 is not None:
        yerel, soz_y, _ = bina.gerek(s1)
    # 34/1: asansör ya da yeri.
    if not bina.asansor and h1 is not None:
        if yerel == "asansor":
            bulgular.append(_bulgu("paiy_elevator_missing",
                                   f"Binada asansör yok ({soz_y}) — {s1.kisa()}; {h1.kisa()}: {_ne(ulusal, neden_u)}",
                                   ONARIM, s1.kural(), "asansör yok"))
        elif ulusal == "asansor":
            bulgular.append(_bulgu("paiy_elevator_missing", f"Binada asansör yok ({soz_u}) — {h1.kisa()}", ONARIM,
                                   h1.kural(), "asansör yok"))
        elif ulusal == "yer":
            bulgular.append(_bulgu("paiy_elevator_missing",
                                   f"Binada asansör yeri yok ({soz_u}; 3 katta asansör yeri bırakılır) — {h1.kisa()}",
                                   ONARIM, h1.kural(), "asansör yeri yok"))
    if h1 is not None:
        if ulusal is None:
            satirlar.append(_satir(h1, y.KAPSAM_DISI, neden_u or soz_u))
        else:
            ne = "asansör" if ulusal == "asansor" else "asansör yeri"
            satirlar.append(_satir(h1, y.GECTI if bina.asansor else y.KALDI,
                                   f"{soz_u}: {ne}; " + ("binada asansör var" if bina.asansor else "binada yok")))
    if s1 is not None:
        if yerel is None:
            satirlar.append(_satir(s1, y.KAPSAM_DISI, soz_y))
        else:
            satirlar.append(_satir(s1, y.GECTI if bina.asansor else y.KALDI,
                                   f"{soz_y}: asansör; " + ("binada asansör var" if bina.asansor else "binada yok")))
    # 34/8: asansör bütün katlara ulaşır.
    zorunlu = ulusal == "asansor"
    if zorunlu and h8 is None:
        b, s = _bilinmiyor("PAİY", "34/8", gun)
        bulgular.append(b)
        satirlar.append(s)
    # Her hâlin ulaşılmasını istediği ama asansörün ulaşmadığı katlar: kaldı ve uzman incelemesi.
    eksik: dict[str, list[str]] = {"PAİY": [], "Sivas": []}
    uzman: dict[str, list[str]] = {"PAİY": [], "Sivas": []}
    ulusal8 = zorunlu and h8 is not None
    if bina.asansor and (ulusal8 or s8 is not None):
        for k in bina.hizmet:
            if k["elevators"]:
                continue
            kim = h8 if ulusal8 else s8
            if ulusal8:
                eksik["PAİY"].append(k["name"])
            if s8 is not None:
                eksik["Sivas"].append(k["name"])
            bulgular.append(_bulgu("paiy_elevator_not_serving",
                                   f"{k['name']}: asansör bu kata ulaşmıyor — {kim.kisa()}", ULASMIYOR, kim.kural(),
                                   f"{k['level']}:ulaşmıyor", uids=k.get("rooms")))
        for k in bina.bodrum:
            if k["elevators"]:
                continue
            u = _ulasilir(k, h8.anahtar["served"]) if ulusal8 else False
            r = _ulasilir(k, s8.anahtar["served"]) if s8 is not None else None
            for ad, x in (("PAİY", u), ("Sivas", r)):
                if x is True:
                    eksik[ad].append(k["name"])
                elif x == "uzman":
                    uzman[ad].append(k["name"])
            icinde = _icinde(k)
            if u is True and r is True:
                bulgular.append(_bulgu("paiy_elevator_not_serving",
                                       f"{k['name']}: asansör bu kata ulaşmıyor (katta {icinde}) — {s8.kisa()}; "
                                       f"{h8.kisa()}", ULASMIYOR, s8.kural(), f"{k['level']}:ulaşmıyor", uids=k.get("rooms")))
            elif u is True and r is False:
                bulgular.append(_bulgu("regulation_conflict",
                                       f"{k['name']} (katta {icinde}): asansör ulaşmıyor; {s8.kisa()} ulaşmasını "
                                       f"aramıyor, {h8.kisa()} arıyor — çelişkide idare karar verir", ULASMIYOR,
                                       f"{s8.kural()} ↔ {h8.kural()}", f"{k['level']}:çelişki", uids=k.get("rooms")))
            elif u is True:
                bulgular.append(_bulgu("paiy_elevator_not_serving",
                                       f"{k['name']}: asansör bu kata ulaşmıyor (katta {icinde}) — {h8.kisa()}",
                                       ULASMIYOR, h8.kural(), f"{k['level']}:ulaşmıyor", uids=k.get("rooms")))
            elif r is True:
                bulgular.append(_bulgu("paiy_elevator_not_serving",
                                       f"{k['name']}: asansör bu kata ulaşmıyor (katta {icinde}) — {s8.kisa()}",
                                       ULASMIYOR, s8.kural(), f"{k['level']}:ulaşmıyor", uids=k.get("rooms")))
            elif u == "uzman" or r == "uzman":
                kim = h8 if u == "uzman" else s8
                bulgular.append(_bulgu("paiy_elevator_not_serving",
                                       f"{k['name']}: yalnız teknik hacim var, asansör ulaşmıyor; 34/8 ortak alanı "
                                       f"olan bodruma ulaşmasını ister, 34/1 teknik hacmi saymaz — {kim.kisa()}",
                                       "Teknik hacimlerin ortak alan sayılıp sayılmadığına idare karar verir; "
                                       "gerekiyorsa kuyuyu bu katta da çizin.", kim.kural(),
                                       f"{k['level']}:teknik", y.UZMAN, uids=k.get("rooms")))
    for ad, hal8, uygulanir in (("PAİY", h8, bina.asansor and ulusal8), ("Sivas", s8, bina.asansor)):
        if hal8 is None:
            continue
        if not uygulanir:
            not_ = "binada asansör yok (34/1)" if not bina.asansor else "asansör zorunlu değil"
            satirlar.append(_satir(hal8, y.KAPSAM_DISI, not_))
        elif eksik[ad]:
            satirlar.append(_satir(hal8, y.KALDI, "asansör ulaşmıyor: " + ", ".join(eksik[ad])))
        elif uzman[ad]:
            satirlar.append(_satir(hal8, y.UZMAN, "yalnız teknik hacimli kat: " + ", ".join(uzman[ad])))
        else:
            satirlar.append(_satir(hal8, y.GECTI, "asansör bütün katlara ulaşıyor"))
    return y.Sonuc(bulgular, satirlar)


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin asansör bulguları: başvuru gününün hâlleriyle; gün yoksa her hâlle denenmiş (``yururluk.denetle``)."""
    veri = facts.get("building") or {}
    if not veri.get("storeys"):
        return y.Sonuc([], [])
    bina = _Bina(veri, _plan(facts.get("permit") or {}))
    sivas = veri.get("local") == "sivas"
    maddeler = ULUSAL + (YEREL if sivas else [])
    return y.denetle(lambda g: kural(bina, sivas, g), maddeler, gun, neden, "asansör kuralı")
