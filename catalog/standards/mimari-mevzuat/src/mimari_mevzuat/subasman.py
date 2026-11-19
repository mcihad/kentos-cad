"""Subasman kotu: Planlı Alanlar İmar Yönetmeliği 4/1-yyy (docs/adr/8153).

- **4/1-yyy (1.10.2017):** subasman kotu (zemin kat taban kotu) binaların zemin kat taban döşemesi üst kotudur; imar
  planında aksine hüküm yoksa ±0,00 kotunun altına düşemez ve +1,20 kotunun üstüne çıkamaz. Binanın ±0,00'ı PAİY'nin
  ±0,00'ıdır, binanın kot aldığı nokta: bina programının binası kendi ±0,00'ını söylüyorsa o (katın ``zero``'su,
  docs/adr/8072), yoksa projeninki; katın kotu taban döşemesinin üst kotudur.
- **Zemin kat:** her binanın (olguların katlarının ``building``'i; binası olmayan katlar projenin) adında Zemin geçen
  katı, yoksa asma kat olmayan katlarından kotu ±0,00'a en yakını. Altına düşen ya da üstüne çıkan kaldı.
- **Planın aksine hükmü** (docs/adr/8158): projenin plan notları ``plinthMost`` (en çok) ve ``plinthLeast`` (en az,
  ±0,00'ın altında eksi) 4/1-yyy'nin +1,20 ve ±0,00'ının yerine geçer; notun satırı (“Plan notu: subasman kotu (en
  çok)”) sonucunu söyler, iki ucu da plan notundaysa 4/1-yyy kapsam dışıdır, biri plandaysa 4/1-yyy öbür ucu okur.

**Okunan olgular** (``facts.building``): katlar (ad, kot, asma kat, binası, mahalleri), plan notları.
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import katman
from . import yururluk as y

MADDELER = [("PAİY", "4/1-yyy")]

#: Planın subasman notları (olguların ``planNotes[].key``'i), satırdaki adları ve başlıkları.
EN_COK, EN_AZ = "plinthMost", "plinthLeast"
NOT_SATIRI = {EN_COK: "Plan notu: subasman kotu (en çok)", EN_AZ: "Plan notu: subasman kotu (en az)"}
NOT_BASLIGI = {EN_COK: "Subasman kotu (en çok)", EN_AZ: "Subasman kotu (en az)"}


def _katla(metin: str) -> str:
    tablo = str.maketrans({"ç": "C", "Ç": "C", "ğ": "G", "Ğ": "G", "ı": "I", "İ": "I", "i": "I", "ö": "O", "Ö": "O",
                           "ş": "S", "Ş": "S", "ü": "U", "Ü": "U"})
    return metin.translate(tablo).upper()


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def zeminler(bina: dict[str, Any]) -> list[dict[str, Any]]:
    """Her binanın zemin katı: adında Zemin geçen katı, yoksa asma kat olmayan katlarından kotu binanın ±0,00'ına en
    yakını; binaların ilk görüldüğü sırayla."""
    gruplar: dict[Any, list[dict[str, Any]]] = {}
    for k in bina.get("storeys") or []:
        gruplar.setdefault(k.get("building"), []).append(k)
    out = []
    for katlar in gruplar.values():
        adli = [k for k in katlar if "ZEMIN" in _katla(k.get("name") or "")]
        adaylar = adli or [k for k in katlar if not k.get("mezzanine")]
        if adaylar:
            out.append(min(adaylar, key=lambda k: abs(float(k["elevation"]) - float(k.get("zero") or 0.0))))
    return out


def _kot(v: float) -> str:
    """Bir kot işaretiyle: “+1.50 m”, “−0.50 m”, “±0.00 m”."""
    if abs(v) < 5e-4:
        return "±0.00 m"
    return f"{'+' if v > 0 else '−'}{katman.metre(abs(v))}"


def _not_satiri(not_: dict[str, Any], durum: str, ne: str) -> dict[str, Any]:
    return {"rule": NOT_SATIRI[not_["key"]], "title": NOT_BASLIGI[not_["key"]], "version": None,
            "gazette": not_["source"], "state": durum, "note": ne}


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin subasman bulguları, 4/1-yyy'nin ve planın subasman notlarının satırları."""
    hal = y.hal("PAİY", "4/1-yyy", gun)
    if hal is None:
        return y.Sonuc([], [])
    notlar = {n["key"]: n for n in bina.get("planNotes") or [] if n["key"] in (EN_COK, EN_AZ)}
    ust_not, alt_not = notlar.get(EN_COK), notlar.get(EN_AZ)
    alt = float(alt_not["value"]) if alt_not else float(hal.anahtar["least"])
    ust = float(ust_not["value"]) if ust_not else float(hal.anahtar["most"])
    zemin = zeminler(bina)
    if not zemin:
        satirlar = [_not_satiri(n, y.KAPSAM_DISI, "zemin kat yok") for n in notlar.values()]
        return y.Sonuc([], [_satir(hal, y.KAPSAM_DISI, "zemin kat yok")] + satirlar)
    bulgular: list[dict[str, Any]] = []
    # Her ucun kaldığı zemin katları: 4/1-yyy'nin ya da planın notunun.
    kalan = {EN_COK: 0, EN_AZ: 0}
    for k in zemin:
        kot = float(k["elevation"]) - float(k.get("zero") or 0.0)
        if alt - 1e-9 <= kot <= ust + 1e-9:
            continue
        asagi = kot < alt
        uc = EN_AZ if asagi else EN_COK
        kalan[uc] += 1
        bag = notlar.get(uc)
        ad = f"“{k.get('name') or k['level']}” zemin katın taban döşemesi (subasman kotu)"
        uids = list(k.get("rooms") or [])[:20]
        if bag is None:
            ne = (f"±0,00'ın {katman.metre(-kot)} altında" if asagi and abs(alt) < 1e-9
                  else f"{_kot(kot)}, en az {_kot(alt)}" if asagi
                  else f"+{katman.metre(kot)}, en çok {_kot(ust)}")
            # Planın öbür ucu yazılıysa 4/1-yyy'nin yalnız bu ucu bağlar.
            ister = ("±0,00 ile +1,20 arasını" if not notlar else "en az ±0,00'ı" if asagi else "en çok +1,20'yi")
            bulgular.append({
                "code": "paiy_plinth_level",
                "message": f"{ad} {ne}: 4/1-yyy {ister} ister (imar planında aksine hüküm yoksa) — {hal.kisa()}",
                "repair": "Zemin katın kotunu ±0,00 ile +1,20 arasına getirin (Proje ayarları › Mimari › Katlar); plan "
                          "aksini söylüyorsa planın notunu Plan notları'na yazın.",
                "uids": uids, "rule": hal.kural(), "_iz": f"subasman:{k['level']}"})
            continue
        plan = katman.PlanNotu(float(bag["value"]), bag["source"])
        ne = f"{_kot(kot)}, en az {_kot(alt)}" if asagi else f"{_kot(kot)}, en çok {_kot(ust)}"
        bulgular.append({
            "code": "paiy_plinth_level",
            "message": f"{ad} {ne}: planın notu (4/1-yyy'nin aksine hükmü) — {plan.kisa()}",
            "repair": "Zemin katın kotunu planın notunun sınırına getirin (Proje ayarları › Mimari › Katlar).",
            "uids": uids, "rule": plan.kural(), "_iz": f"subasman:{k['level']}:plan:{bag['value']}"})
    kotlar = ", ".join(_kot(float(k["elevation"]) - float(k.get("zero") or 0.0)).removesuffix(" m") for k in zemin)
    satirlar = []
    for uc, n in notlar.items():
        sinir = f"en çok {_kot(ust)}" if uc == EN_COK else f"en az {_kot(alt)}"
        satirlar.append(_not_satiri(n, y.KALDI if kalan[uc] else y.GECTI,
                                    f"{kalan[uc]} zemin kat {sinir} dışında" if kalan[uc]
                                    else f"zemin kat {kotlar} m ({sinir})"))
    if len(notlar) == 2:
        durum = _satir(hal, y.KAPSAM_DISI, "subasman kotu parselin plan notunda")
    else:
        # Planın notu olmayan uç 4/1-yyy'nin.
        uclar = [uc for uc in (EN_AZ, EN_COK) if uc not in notlar]
        kac = sum(kalan[uc] for uc in uclar)
        if kac:
            sinir = ("±0,00 ile +1,20" if not notlar else "en az ±0,00" if EN_AZ in uclar else "en çok +1,20")
            durum = _satir(hal, y.KALDI, f"{kac} zemin katın subasmanı {sinir} dışında")
        else:
            durum = _satir(hal, y.GECTI, f"zemin kat {kotlar} m")
    return y.Sonuc(bulgular, [durum] + satirlar)


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin subasman bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "subasman kuralı")
