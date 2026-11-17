"""Kuralların yürürlüğü (docs/adr/8078; plan/bim-plan.md BIM-13): bir maddenin başvuru gününde uygulanan hâli.

Maddelerin hâlleri ``data/surumler.json``'dadır: metni, Resmî Gazete'si, yürürlüğe girdiği ve yerini bıraktığı gün,
geçiş hükmü ve kuralın o hâli okurken kullandığı anahtarlar (bodrumu neyin saydırdığı gibi). Hangi günün sayıldığını
geçiş hükmü söyler: Planlı Alanlar İmar Yönetmeliği'nin 1.7.2026 değişikliğinde Geçici 3/11, yürürlükten önce idareye
başvurusu yapılmış yapılara önceki hâlin uygulanacağını söyler. Sayılan gün projenin künyesindeki başvuru tarihidir
(``basvuru_tarihi``).

- **Gün yoksa ya da okunamıyorsa** kural, kural setinin kapsadığı her hâlle ayrı ayrı denenir (``denetle``): sonuç
  hepsinde aynıysa en yeni hâliyle verilir; değilse bulgu "eksik veri"dir ve hangi gün hangi sonucu verdiğini söyler.
  KentOS tahmin etmez.
- **Gün kural setinin bildiği ilk hâlden önceyse** o günün metni KentOS'ta yoktur: bulgu "uzman incelemesi"dir.
"""

from __future__ import annotations

import json
import re
from collections.abc import Callable
from dataclasses import dataclass, field
from datetime import date, timedelta
from functools import lru_cache
from pathlib import Path
from typing import Any

#: Bulgunun durumu: ``None`` kaldı; öbürleri ``ArchAuditFinding.state``'in değerleri.
EKSIK_VERI = "missingData"
UZMAN = "expertReview"

#: Kuralın özet satırındaki sonuç.
GECTI, KALDI, KAPSAM_DISI = "passed", "failed", "outOfScope"

_ADLAR = {"PAİY": "PAİY", "Sivas": "Sivas İmar Yön."}


@dataclass(frozen=True)
class Hal:
    """Bir maddenin bir hâli: yönetmeliği, maddesi, yürürlükte olduğu günler, Resmî Gazete'si ve okuma anahtarları."""

    yonetmelik: str
    madde: str
    baslik: str
    kimlik: str
    bas: date
    bit: date | None
    etiket: str
    rg: str
    anahtar: dict[str, Any] = field(default_factory=dict, compare=False, hash=False)

    def gecerli(self, gun: date) -> bool:
        """Bu hâl ``gun``de yürürlükte mi."""
        return self.bas <= gun and (self.bit is None or gun <= self.bit)

    def kisa(self, bent: str | None = None) -> str:
        """İletinin sonundaki adı: “PAİY 34/1 (1.7.2026)”; yerel yönetmeliğin “Sivas 34/1”. ``bent`` fıkranın bendi:
        “PAİY 39/1-c (13.8.2025)”."""
        madde = f"{self.madde}-{bent}" if bent else self.madde
        if self.yonetmelik == "PAİY":
            return f"PAİY {madde} ({self.etiket})"
        return f"{self.yonetmelik} {madde}"

    def kural(self, bent: str | None = None) -> str:
        """Bulgunun ``rule``'u: “PAİY 34/1 · RG 1.7.2026/33297”; bendiyle “PAİY 39/1-c · RG 13.8.2025/32985”."""
        madde = f"{self.madde}-{bent}" if bent else self.madde
        return f"{_ADLAR.get(self.yonetmelik, self.yonetmelik)} {madde} · RG {self.rg}"


@lru_cache(maxsize=1)
def veri() -> dict[str, Any]:
    """Maddelerin hâlleri (``data/surumler.json``)."""
    return json.loads((Path(__file__).parent / "data" / "surumler.json").read_text(encoding="utf-8"))


def haller(yonetmelik: str, madde: str) -> list[Hal]:
    """Maddenin bilinen hâlleri, eskiden yeniye."""
    for m in veri()["articles"]:
        if m["regulation"] == yonetmelik and m["article"] == madde:
            out = []
            for v in m["versions"]:
                ozel = {k: x for k, x in v.items() if k not in {"id", "from", "until", "label", "gazette", "text",
                                                              "source"}}
                out.append(
                    Hal(
                        yonetmelik=yonetmelik,
                        madde=madde,
                        baslik=m["title"],
                        kimlik=v["id"],
                        bas=date.fromisoformat(v["from"]),
                        bit=date.fromisoformat(v["until"]) if v["until"] else None,
                        etiket=v["label"],
                        rg=v["gazette"],
                        anahtar=ozel,
                    )
                )
            return sorted(out, key=lambda h: h.bas)
    raise KeyError(f"{yonetmelik} {madde} kural setinde yok")


def hal(yonetmelik: str, madde: str, gun: date) -> Hal | None:
    """Maddenin ``gun``de yürürlükteki hâli; o gün bilinmiyorsa yok."""
    return next((h for h in haller(yonetmelik, madde) if h.gecerli(gun)), None)


def ilk(yonetmelik: str, madde: str) -> date:
    """Maddenin kural setindeki ilk hâlinin günü."""
    return haller(yonetmelik, madde)[0].bas


def gun_degeri(metin: str | date | None) -> date | None:
    """``GG.AA.YYYY``, ``G.A.YYYY``, ``GG/AA/YYYY`` ya da ``YYYY-AA-GG``; olmayan bir gün okunmaz."""
    if metin is None:
        return None
    if isinstance(metin, date):
        return metin
    t = str(metin).strip()
    m = re.fullmatch(r"(\d{1,2})[./](\d{1,2})[./](\d{4})", t)
    if m:
        g, a, y = int(m.group(1)), int(m.group(2)), int(m.group(3))
    else:
        m = re.fullmatch(r"(\d{4})-(\d{1,2})-(\d{1,2})", t)
        if not m:
            return None
        y, a, g = int(m.group(1)), int(m.group(2)), int(m.group(3))
    try:
        return date(y, a, g)
    except ValueError:
        return None


def yaz(gun: date) -> str:
    """Bir gün iletide: “2.7.2026”."""
    return f"{gun.day}.{gun.month}.{gun.year}"


def basvuru(permit: dict[str, Any] | None) -> tuple[date | None, str | None]:
    """Künyedeki başvuru tarihi ve okunamadıysa nedeni (``basvuru_tarihi``)."""
    metin = str(((permit or {}).get("particulars") or {}).get("basvuru_tarihi") or "").strip()
    if not metin:
        return None, None
    gun = gun_degeri(metin)
    if gun is None:
        return None, f"Başvuru tarihi “{metin}” okunamadı: GG.AA.YYYY yazın (02.07.2026)."
    return gun, None


def adaylar(maddeler: list[tuple[str, str]]) -> list[date]:
    """Gün bilinmezken denenecek günler: maddelerin her hâlinin ilk günü, ulusal maddelerin hepsinin bilindiği ilk
    günden başlayarak (daha eskisinin metni kural setinde yok)."""
    ulusal = [m for m in maddeler if m[0] == "PAİY"] or maddeler
    kapsam = max(ilk(*m) for m in ulusal)
    gunler = {h.bas for m in maddeler for h in haller(*m) if h.bas >= kapsam}
    return sorted(gunler | {kapsam})


@dataclass
class Sonuc:
    """Bir kuralın bir gündeki sonucu: bulguları ve maddelerinin özet satırları."""

    bulgular: list[dict[str, Any]]
    satirlar: list[dict[str, Any]]

    def iz(self) -> tuple:
        """Karşılaştırma için: her bulgunun kodu, nesneleri, durumu ve ne istediği (``_iz``; yoksa maddeyi söyleyen
        sonu olmadan iletisi). Hâlin günü ve bağlayan madde sayılmaz: aynı şeyi isteyen iki hâl kullanıcı için aynıdır;
        bulgusu olmayan iki sonuç da (biri geçti, öbürü kapsam dışı olsa da)."""
        return tuple(sorted((f["code"], tuple(f.get("uids") or ()), f.get("state") or "",
                             f.get("_iz") or _ozu(f["message"])) for f in self.bulgular))


def _ozu(ileti: str) -> str:
    """İletinin maddeyi söyleyen sonu olmadan: aynı ölçü iki hâlde aynı söylenir."""
    return ileti.split(" — ")[0]


def denetle(kural: Callable[[date], Sonuc], maddeler: list[tuple[str, str]], gun: date | None,
            neden: str | None, konu: str) -> Sonuc:
    """``kural``'ı başvuru gününün hâlleriyle çalıştırır; gün yoksa her aday günle dener: sonuç hepsinde aynıysa en
    yenisini verir, değişiyorsa tek bir "eksik veri" bulgusu. ``neden`` günün okunamama nedeni (bulgu onunla başlar);
    ``konu`` iletideki kuralın adı (“asansör kuralı”)."""
    if gun is not None:
        return kural(gun)
    gunler = adaylar(maddeler)
    sonuclar = [(g, kural(g)) for g in gunler]
    ilk_iz = sonuclar[0][1].iz()
    if all(s.iz() == ilk_iz for _, s in sonuclar):
        return sonuclar[-1][1]
    parcalar = []
    for i, (g, s) in enumerate(sonuclar):
        bitis = sonuclar[i + 1][0] - timedelta(days=1) if i + 1 < len(sonuclar) else None
        aralik = f"{yaz(g)}–{yaz(bitis)}" if bitis else f"{yaz(g)} ve sonrası"
        ne = "; ".join(_ozu(f["message"]) for f in s.bulgular) or "bulgu yok"
        parcalar.append(f"{aralik} başvurusunda {ne}")
    bas = neden or "Başvuru tarihi künyede yok:"
    ileti = f"{bas} {konu} başvuru gününe göre değişiyor — " + " · ".join(parcalar)
    son = sonuclar[-1][1]
    satirlar = [dict(r, state=EKSIK_VERI, version=None, gazette=None) for r in son.satirlar]
    return Sonuc([_eksik(ileti, [h for h in maddeler])], satirlar)


def parcalarla(parcalar: list[Any], kural: Callable[[Any, date], Sonuc], maddeler: list[tuple[str, str]],
               gun: date | None, neden: str | None, konu: str,
               nesneler: Callable[[Any], list[str]]) -> list[Sonuc]:
    """``kural(parca, gun)``'ı her parça (bir kat, bir kapı) için ayrı ``denetle``r: gün yokken sonucu güne göre
    değişmeyen parçanın bulguları en yeni hâlle kalır, değişeninki o parçanın tek "eksik veri" bulgusudur ve parçanın
    nesnelerini (``nesneler``) seçer. Bir kapının ölçüsü güne bağlı diye öbür kapıların bulguları saklanmaz
    (docs/adr/8083). Her parçanın sonucu sırayla; satırlarını ``birlesik`` birleştirir."""
    out = []
    for p in parcalar:
        s = denetle(lambda g, p=p: kural(p, g), maddeler, gun, neden, konu)
        for b in s.bulgular:
            if b["code"] == "regulation_date_needed" and not b["uids"]:
                b["uids"] = list(nesneler(p))
        out.append(s)
    return out


def birlesik(sonuclar: list[Sonuc]) -> list[dict[str, Any]]:
    """Parçaların satırları madde madde, ilk görüldükleri sırayla: her durumun kaç parçada çıktığı (``sayilar``);
    hâli bütün parçalarda aynıysa o, değilse yok. Durum: bir parça güne bağlıysa eksik veri, değilse bir parça kaldıysa
    kaldı, uzman incelemesi, geçti; hiçbirinde okunmadıysa kapsam dışı. Notu kural yazar (``note`` boş)."""
    sira: list[str] = []
    satirlar: dict[str, dict[str, Any]] = {}
    for s in sonuclar:
        for r in s.satirlar:
            k = r["rule"]
            if k not in satirlar:
                sira.append(k)
                satirlar[k] = {"rule": k, "title": r["title"], "version": r["version"], "gazette": r["gazette"],
                               "sayilar": {}}
            m = satirlar[k]
            if (m["version"], m["gazette"]) != (r["version"], r["gazette"]):
                m["version"] = m["gazette"] = None
            m["sayilar"][r["state"]] = m["sayilar"].get(r["state"], 0) + 1
    out = []
    for k in sira:
        m = satirlar[k]
        n = m["sayilar"]
        durum = next((d for d in (EKSIK_VERI, KALDI, UZMAN, GECTI) if n.get(d)), KAPSAM_DISI)
        out.append(dict(m, state=durum, note=""))
    return out


def _eksik(ileti: str, maddeler: list[tuple[str, str]]) -> dict[str, Any]:
    kural = ", ".join(f"{_ADLAR.get(y, y)} {m}" for y, m in maddeler) or None
    return {
        "code": "regulation_date_needed",
        "message": ileti,
        "repair": "Ruhsat takımı penceresinin Künye bölümünde Başvuru tarihi'ni yazın (GG.AA.YYYY): kural seti o güne "
                  "göre seçilir.",
        "uids": [],
        "rule": kural,
        "state": EKSIK_VERI,
    }
